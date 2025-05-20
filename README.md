# count

command line applications for recursive line counting in files

### todo:
- [x] recursive counting of lines in files
- [x] specifying the file/directory to be counted
- [x] exclude files/directories from counting
- [x] highlighting/ignoring certain file formats
- [ ] specifying how rows are counted:
    - [ ] counting all lines in files
    - [ ] counting lines without line breaks
    - [ ] specifying that different formats should be counted as one
    - [ ] regular expression specification
- [ ] result sorting in specific way
- [x] print result as tree (-t=1 prints dirs in curr dir separately)
- [x] loading message
- [x] help message

### get

requires [go](https://go.dev/doc/install) for building executable file

```bash
git clone git@github.com:Stasenko-Konstantin/count.git 
cd count
./build.sh     # requires sudo for cp executable file to /bin
               # reopen terminal
count -h                  
```

### usage

```bash
NAME:
   count - counter of text files lines 

USAGE:
   count [GLOBAL OPTIONS]

GLOBAL OPTIONS:
   --paths string, -p string [ --paths string, -p string ]        list of paths to count (default: ".")
   --ext string, -e string                                        file extension for exclusive counting
   --excludes string, -E string [ --excludes string, -E string ]  list of paths/extensions to exclude from counting
   --tree int, -t int                                             dir level (default: 0)
   --help, -h 
```
