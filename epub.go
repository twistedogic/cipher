package main

import (
	"archive/zip"
	"fmt"
	"io"

	htmltomarkdown "github.com/JohannesKaufmann/html-to-markdown/v2"
	"github.com/charmbracelet/glamour"
	"github.com/taylorskalyo/goreader/epub"
)

func EpubToMarkdown(path string) error {
	zr, err := zip.OpenReader(path)
	if err != nil {
		return err
	}
	rc, err := epub.OpenReader(path)
	if err != nil {
		return err
	}
	defer rc.Close()
	for _, book := range rc.Rootfiles {
		for _, item := range book.Items {
			if item.ID == "ncx" {
				r, err := item.Open()
				if err != nil {
					return err
				}
				defer r.Close()
				_, err = io.ReadAll(r)
				if err != nil {
					return err
				}
				// fmt.Println(string(b))
			}
		}
		for _, itemRef := range book.Spine.Itemrefs {
			fmt.Println(itemRef.IDREF)
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
