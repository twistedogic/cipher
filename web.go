package main

import (
	"net/http"
	"time"

	htmltomarkdown "github.com/JohannesKaufmann/html-to-markdown/v2"
	"github.com/PuerkitoBio/goquery"
)

func FromURL(link string) (*Document, error) {
	res, err := http.Get(link)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()
	doc, err := goquery.NewDocumentFromReader(res.Body)
	if err != nil {
		return nil, err
	}
	title := doc.Find("head > title").First().Text()
	content, err := doc.Html()
	if err != nil {
		return nil, err
	}
	md, err := htmltomarkdown.ConvertString(content)
	if err != nil {
		return nil, err
	}
	return &Document{
		Metadata: Metadata{
			Title: title,
			Name:  title,
			Path:  link,
			Type:  WebType,
			Date:  time.Now(),
		},
		Content: []byte(md),
	}, nil
}
