package main

import "os"

func main() {
	if err := EpubToMarkdown(os.Args[1]); err != nil {
		panic(err)
	}
}
