# Graphviz DOT Binary

## Version
- **Graphviz version:** 2.43.0 (0)

## Description
This repository contains a standalone version of the `dot` binary from Graphviz ( https://graphviz.org/download/source/ ), along with its required shared libraries. 
The purpose of this setup is to allow execution of the `dot` tool without requiring system-wide installation of Graphviz dependencies.

## Usage
To execute the `dot` binary using the provided shared libraries, set the `LD_LIBRARY_PATH` environment variable to the `lib` directory before running `dot`:

```sh
export LD_LIBRARY_PATH=$(pwd)/lib:$LD_LIBRARY_PATH
./dot -V


## Structure

./dot            # The Graphviz DOT binary
./lib/           # Directory containing required shared libraries


## License

Refer to the original Graphviz license for terms of use.
Excerpt:

If you distribute the binaries without modification, you do not need to provide the source code, but you should reference the original source.

https://graphviz.org/download/source/
