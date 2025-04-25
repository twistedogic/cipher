package main

import (
	"encoding/xml"
	"fmt"
	"io"

	htmltomarkdown "github.com/JohannesKaufmann/html-to-markdown/v2"
	"github.com/charmbracelet/glamour"
	"github.com/taylorskalyo/goreader/epub"
)

type Chapter struct {
	Title string
	Path  string
	Src   string
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

func (n Nav) Flatten() []Chapter {
	if len(n.Points) == 0 {
		return []Chapter{{Path: n.Title, Title: n.Title, Src: n.Content.Src}}
	}
	chapters := make([]Chapter, 0, len(n.Points))
	for _, p := range n.Points {
		children := p.Flatten()
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
		for _, chap := range c.Flatten() {
			fmt.Printf("%#v\n", chap)
		}
	}
	itemMap := make(map[string]epub.Item)
	for _, book := range rc.Rootfiles {
		for _, item := range book.Items {
			itemMap[item.HREF] = item
		}
		for _, itemRef := range book.Spine.Itemrefs {
			r, err := itemRef.Open()
			if err != nil {
				return err
			}
			defer r.Close()
			b, err := io.ReadAll(r)
			if err != nil {
				return err
			}
			md, err := htmltomarkdown.ConvertString(string(b))
			if err != nil {
				return err
			}
			_, err = glamour.Render(md, "dark")
			if err != nil {
				return err
			}
		}
	}
	return nil
}
