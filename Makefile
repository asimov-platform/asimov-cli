READMER = readmer

all: README.md

README.md: .config/readmer/README.md.liquid
	$(READMER) render $< > $@

readmes: README.md

clean:

maintainer-clean:

.PHONY: all readmes clean maintainer-clean
.SECONDARY:
.SUFFIXES:
