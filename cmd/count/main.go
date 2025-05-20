package main

import (
	"context"
	"fmt"
	"github.com/urfave/cli/v3"
	"os"
	pathl "path"
	"slices"
	"strings"
	"unicode"
	"unicode/utf8"
)

const textThreshold = 0.8 // TODO very strange stuff

var (
	paths    []string
	ext      string
	excludes []string
	tree     uint8
)

type ptree struct {
	paths    []string
	nodes    []*ptree
	currpath string
}

func main() {
	(&cli.Command{
		Name:      "count",
		Usage:     "counter of text files lines ",
		UsageText: "count [GLOBAL OPTIONS]",
		Action:    mainAction,
		Flags: []cli.Flag{
			&cli.StringSliceFlag{
				Name:    "paths",
				Aliases: []string{"p"},
				Value:   []string{"."},
				Usage:   "list of paths to count",
				Action: func(ctx context.Context, command *cli.Command, pathsval []string) error {
					paths = pathsval
					return nil
				},
			},
			&cli.StringFlag{
				Name:    "ext",
				Aliases: []string{"e"},
				Usage:   "file extension for exclusive counting",
				Action: func(ctx context.Context, command *cli.Command, extval string) error {
					ext = extval
					return nil
				},
			},
			&cli.StringSliceFlag{
				Name:    "excludes",
				Aliases: []string{"E"},
				Usage:   "list of paths/extensions to exclude from counting",
				Action: func(ctx context.Context, command *cli.Command, excludesval []string) error {
					excludes = excludesval
					return nil
				},
			},
			&cli.IntFlag{
				Name:    "tree",
				Aliases: []string{"t"},
				Value:   0,
				Usage:   "dir level",
				Action: func(ctx context.Context, command *cli.Command, treeval int) error {
					if treeval > 255 {
						tree = 255
					} else if treeval > 0 {
						tree = uint8(treeval)
					}
					return nil
				},
			},
		},
	}).Run(context.Background(), os.Args)
}

func mainAction(_ context.Context, _ *cli.Command) error {
	currpath, err := os.Getwd()
	if err != nil {
		return err
	}
	if len(paths) == 0 {
		paths = []string{currpath}
	}
	t := &ptree{}
	if tree == 0 {
		t.currpath = currpath
	}
	tdeep := int(tree)
	if tree > 0 {
		tdeep += 1
	}
	if err := mktree(t, paths, tdeep); err != nil {
		return err
	}
	if err := walk(t, tdeep); err != nil {
		return err
	}
	fmt.Println("\ndone!")
	return nil
}

func mktree(tree *ptree, paths []string, tdeep int) error {
	if tdeep == -1 {
		return nil
	}
	for _, p := range paths {
		if p == "." {
			wd, err := os.Getwd()
			if err != nil {
				return err
			}
			p = wd
		}
		if !isNeedExcludePath(p) {
			s, err := os.Stat(p)
			if err != nil {
				return err
			}
			if s.IsDir() && tdeep != 0 {
				t := &ptree{
					currpath: p,
				}
				es, err := os.ReadDir(p)
				if err != nil {
					return err
				}
				var ps []string
				for _, e := range es {
					ps = append(ps, pathl.Join(t.currpath, e.Name()))
				}
				if err := mktree(t, ps, tdeep-1); err != nil {
					return err
				}
				tree.nodes = append(tree.nodes, t)
			}
			tree.paths = append(tree.paths, p)
		}
	}
	return nil
}

func walk(tree *ptree, tdeep int) error {
	if tdeep == 0 && len(tree.paths) > 0 && tree.currpath != "" {
		if err := printtree(tree.currpath, tree.paths); err != nil {
			fmt.Println(err.Error())
			return err
		}
	}
	if len(tree.nodes) == 0 {
		return nil
	}
	if tdeep > 0 {
		for _, n := range tree.nodes {
			if err := walk(n, tdeep-1); err != nil {
				return err
			}
		}
	}
	return nil
}

func printtree(currpath string, paths []string) error {
	findex, err := mkindex(paths)
	if err != nil {
		return err
	}
	if len(findex) == 0 {
		return nil
	}
	fmt.Printf("\ncounting %d files in %s...\n", len(findex), currpath)
	res, err := count(findex)
	if err != nil {
		return err
	}
	for e, c := range res {
		if e == "." || (ext != "" && e != ext) {
			continue
		}
		fmt.Printf("%s\t\t%d\n", e, c)
	}
	return nil
}

func isNeedExcludePath(path string) bool {
	fext := pathl.Ext(path)
	file := pathl.Base(path)
	return strings.HasPrefix(file, ".") ||
		slices.Contains(excludes, file) ||
		slices.Contains(excludes, fext)
}

func mkindex(paths []string) ([]string, error) {
	var findex []string
	for _, p := range paths {
		s, err := os.Stat(p)
		if err != nil {
			return nil, err
		}
		if !s.IsDir() {
			if isNeedExcludePath(p) || isNeedFilterByExt(p) {
				continue
			}
			findex = append(findex, p)
			continue
		}
		if isNeedExcludePath(p) {
			continue
		}
		es, err := os.ReadDir(p)
		if err != nil {
			return nil, err
		}
		for _, e := range es {
			if strings.HasPrefix(e.Name(), ".") {
				continue
			}
			ep := pathl.Join(p, e.Name())
			s, err := os.Stat(ep)
			if err != nil {
				return nil, err
			}
			if s.IsDir() {
				if isNeedExcludePath(ep) {
					continue
				}
				fi, err := mkindex([]string{ep})
				if err != nil {
					return nil, err
				}
				findex = append(findex, fi...)
			}
			if isNeedExcludePath(ep) || isNeedFilterByExt(ep) {
				continue
			}
			if !isTextFile(ep) {
				continue
			}
			findex = append(findex, ep)
		}
	}
	return findex, nil
}

func isNeedFilterByExt(path string) bool {
	fext := pathl.Ext(path)
	return ext != "" && fext != ext
}

func count(findex []string) (map[string]int, error) {
	res := make(map[string]int)
	for _, path := range findex {
		rext := "."
		if e := pathl.Ext(path); e != "" {
			rext = e
		}
		fl, err := countFileLines(path)
		if err != nil {
			continue
		}
		res[rext] += fl
	}
	return res, nil
}

func countFileLines(path string) (int, error) {
	f, err := os.ReadFile(path)
	if err != nil {
		return 0, err
	}
	res := len(strings.Split(string(f), "\n"))
	return res, nil
}

func isTextFile(path string) bool {
	f, err := os.ReadFile(path)
	if err != nil {
		return false
	}
	if len(f) == 0 {
		return false
	}
	if !utf8.Valid(f) {
		return false
	}
	var printable int
	for _, r := range string(f) {
		if unicode.IsPrint(r) || unicode.IsSpace(r) {
			printable++
		}
	}
	return float64(printable)/float64(len(f)) > textThreshold
}
