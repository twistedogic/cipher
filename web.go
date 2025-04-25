package main

import (
	"fmt"
	"net/http"

	htmltomarkdown "github.com/JohannesKaufmann/html-to-markdown/v2"
)

func SiteToMarkdown(link string) (*Content, error) {
	res, err := http.Get(link)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()
	md, err := htmltomarkdown.ConvertReader(res.Body)
	if err != nil {
		return nil, err
	}
	fmt.Println(string(md))
	return nil, nil
}
