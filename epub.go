package main

import (
	"encoding/xml"
	"fmt"
	"io"
	"net/url"

	htmltomarkdown "github.com/JohannesKaufmann/html-to-markdown/v2"
	"github.com/PuerkitoBio/goquery"
	"github.com/charmbracelet/glamour"
	"github.com/taylorskalyo/goreader/epub"
)

type Chapter struct {
	Title      string
	Path       string
	Src        string
	prev, next *Chapter
	Content    []byte
}

func (c *Chapter) String() string {
	if c == nil {
		return "No content."
	}
	md, err := htmltomarkdown.ConvertString(string(c.Content))
	if err != nil {
		return string(c.Content)
	}
	if pretty, err := glamour.Render(md, "dark"); err == nil {
		return pretty
	}
	return string(c.Content)
}

type NavMap struct {
	Points []Nav `xml:"navMap>navPoint"`
}

type Nav struct {
	Title   string `xml:"navLabel>text"`
	Content struct {
		Src string `xml:"src,attr"`
	} `xml:"content"`
	Points []Nav `xml:"navPoint"`
}

func readContent(src string, itemMap map[string]epub.Item) ([]byte, error) {
	u, err := url.Parse(src)
	if err != nil {
		return nil, err
	}
	item, ok := itemMap[u.Path]
	if !ok {
		return nil, fmt.Errorf("item not found for %q", u.Path)
	}
	r, err := item.Open()
	if err != nil {
		return nil, err
	}
	defer r.Close()
	if u.Fragment == "" {
		return io.ReadAll(r)
	}
	doc, err := goquery.NewDocumentFromReader(r)
	if err != nil {
		return nil, err
	}
	html, err := doc.Find("#" + u.Fragment).First().Html()
	return []byte(html), err
}

func (n Nav) toChapter() *Chapter {
	return &Chapter{Path: n.Title, Title: n.Title, Src: n.Content.Src}
}

func (n Nav) flatten() []*Chapter {
	chapters := make([]*Chapter, 0, len(n.Points)+1)
	chapters = append(chapters, n.toChapter())
	for _, p := range n.Points {
		children := p.flatten()
		for i, child := range children {
			children[i].Path = n.Title + " > " + child.Path
		}
		chapters = append(chapters, children...)
	}
	return chapters
}

func parseTOC(rc *epub.ReadCloser) ([]Nav, error) {
	toc := []Nav{}
	for _, book := range rc.Rootfiles {
		for _, item := range book.Items {
			if item.ID == "ncx" {
				r, err := item.Open()
				if err != nil {
					return nil, err
				}
				defer r.Close()
				b, err := io.ReadAll(r)
				if err != nil {
					return nil, err
				}
				m := NavMap{}
				if err := xml.Unmarshal(b, &m); err != nil {
					return nil, err
				}
				toc = append(toc, m.Points...)
			}
		}
	}
	return toc, nil
}

func mapItems(rc *epub.ReadCloser) map[string]epub.Item {
	itemMap := make(map[string]epub.Item)
	for _, book := range rc.Rootfiles {
		for _, item := range book.Items {
			itemMap[item.HREF] = item
		}
	}
	return itemMap
}

func EpubToMarkdown(path string) error {
	rc, err := epub.OpenReader(path)
	if err != nil {
		return err
	}
	defer rc.Close()
	toc, err := parseTOC(rc)
	if err != nil {
		return err
	}
	for _, c := range toc {
		for _, chap := range c.flatten() {
			fmt.Println(chap.Title, chap)
		}
	}
	return nil
}
