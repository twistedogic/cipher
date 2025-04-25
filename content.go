package main

import "time"

type SourceType uint

const (
	Web SourceType = iota
	Book
)

type Source struct {
	Type SourceType
	Name string
	Path string
	Date time.Time
}

type Content struct {
	Title    string
	Path     string
	Metadata map[string]string
	Content  []byte
	Source   Source
}
