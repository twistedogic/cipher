package main

import "time"

const (
	WebType  = "web"
	BookType = "book"

	titleKey = "title"
	nameKey  = "name"
	pathKey  = "path"
	typeKey  = "type"
	dateKey  = "date"
)

type Metadata struct {
	Title string
	Name  string
	Path  string
	Type  string
	Date  time.Time
}

func fromMap(m map[string]string) Metadata {
	date, _ := time.Parse(time.DateOnly, m[dateKey])
	return Metadata{
		Title: m[titleKey],
		Name:  m[nameKey],
		Path:  m[pathKey],
		Type:  m[typeKey],
		Date:  date,
	}
}

func (m Metadata) toMap() map[string]string {
	return map[string]string{
		titleKey: m.Title,
		nameKey:  m.Name,
		pathKey:  m.Path,
		typeKey:  m.Type,
		dateKey:  m.Date.UTC().Format(time.DateOnly),
	}
}

type Document struct {
	Metadata Metadata
	Content  []byte
}
