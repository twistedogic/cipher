package main

import (
	"fmt"
	"io"
	"os"

	htmltomarkdown "github.com/JohannesKaufmann/html-to-markdown/v2"
	"github.com/charmbracelet/glamour"
	"github.com/taylorskalyo/goreader/epub"
)

func EpubToMarkdown(path string) error {
	rc, err := epub.OpenReader(path)
	if err != nil {
		return err
	}
	defer rc.Close()
	for _, book := range rc.Rootfiles {
		fmt.Println(book.Metadata)
		for _, item := range book.Manifest.Items {
			if item.MediaType == "application/x-dtbncx+xml" {
				r, err := item.Open()
				if err != nil {
					return err
				}
				defer r.Close()
				b, err := io.ReadAll(r)
				if err != nil {
					return err
				}
				fmt.Println(string(b))
			}
		}
		for _, itemRef := range book.Spine.Itemrefs {
			r, err := itemRef.Open()
			if err != nil {
				return err
			}
			defer r.Close()
			md, err := htmltomarkdown.ConvertReader(r)
			if err != nil {
				return err
			}
			if _, err := glamour.RenderBytes(md, "dark"); err != nil {
				return err
			}
		}
	}
	return nil
}

func main() {
	if err := EpubToMarkdown(os.Args[1]); err != nil {
		panic(err)
	}
}
