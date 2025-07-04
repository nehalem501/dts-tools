# DTS Tools

DTS tools contains various command line utilities to work with files from cinema DTS disc files.
It can show file metadata and recreate regular DTS files from XD10 ingested content (both feature audio and trailers).

# Requirements

To build this program you need a working rust toolchain installed on your computer (you can install it from [here](https://rust-lang.org/)).

Then just build the source by running `cargo`.

#### Debug build
```
cargo build
```
The built executable will be inside `target/debug`.

#### Release build
```
cargo build --release
```
The built executable will be inside `target/release`.

# Usage

```
Usage: dts-tools [OPTIONS] <COMMAND>

Commands:
  info
  extract
  help     Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose
  -h, --help     Print help
  -V, --version  Print version
```
## Info

Show DTS files metadata.

```
Usage: dts-tools info [OPTIONS] [FILE]...

Arguments:
  [FILE]...

Options:
  -v, --verbose
  -h, --help     Print help
```
#### Display contents of CD

##### Display metadata for ISO disk image
```
dts-tools info path/to/file.iso
```

##### Display metadata for CD drive
```
dts-tools info path/to/cd/drive
```

##### Display metadata for directory containing CD file structure
```
dts-tools info path/to/directory
```
It expects this kind of structure:
```
├── DTS.EXE
└── DTS
    ├── R1T5.AUD
    ├── R2T5.AUD
    └── R3T5.AUD
```

#### Display contents of directory containing XD10 files

```
dts-tools info path/to/directory
```

## Extract

Extract DTS files from XD10 files.

```
Usage: dts-tools extract [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>
  <OUTPUT>

Options:
      --feature-name <FEATURE_NAME>
  -v, --verbose
      --feature-id <FEATURE_ID>
      --trailer-names <TRAILER_NAMES>...
      --trailer-ids <TRAILER_IDS>...
  -h, --help
```

The tool will scan all files in the input directory to find all associated sound files.
The provided output directory will be filled with all movie and / or trailer files requested.
```
├── DTS.EXE
└── DTS
    ├── R14T5.AUD
    ├── R14TRLR.TXT
    ├── R1T5.AUD
    ├── R2T5.AUD
    └── R3T5.AUD
```

#### Extract movie soundtrack using feature id

```
dts-tools extract --feature-id 12345 path/to/xd10/data path/to/output
```

#### Extract movie soundtrack using feature name

```
dts-tools extract --feature-name 'My Movie' path/to/xd10/data path/to/output
```

#### Extract trailer soundtrack using trailer id

```
dts-tools extract --trailer-ids=123 path/to/xd10/data path/to/output
```
You can also provide multiple ids separated by `,` like this:
`--trailer-ids=421,856,9031`

#### Extract trailer soundtrack using trailer name

```
dts-tools extract --trailer-names=MYTRAILER path/to/xd10/data path/to/output
```
You can also provide multiple names separated by `,`like this:
`--trailer-names=TRAILER1,TRAILER2`

#### Extract both trailers and feature soundtracks

```
dts-tools extract -trailer-ids=123,456 --feature-id 12345 path/to/xd10/data path/to/output
```

# Roadmap

Planned features are:

- Read Linux formatted ext2/3/4 drives directly from operating systems not supporting this filesystem (such as Windows or macOS).
- Read from backup disc images in raw and compressed squashfs formats.
- Repack trailer files.
- Create ISO disc images that you can burn directly to a CD
- Export metadata to JSON files.

# Thanks

Special thanks to all the 35mm cinema related forums where users have documented a lot of things about DTS files over the years.

# License

This software is licensed under the [GNU General Public License 3.0](https://www.gnu.org/licenses/gpl-3.0.txt),
a copy of which can be found in the [LICENSE](LICENSE) file.