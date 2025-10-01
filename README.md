# Apsis
Apsis is a prototype server using the _Encoding for Robust Immutable Storage_ [spec](https://eris.codeberg.page/spec/).
## How it works
Files or JSON data encoded using ERIS are split up into encrypted blocks and stored in the database. Individual blocks are also advertised via the Bittorrent Mainline DHT. When Apsis is missing a block, it performs a lookup on the DHT for the missing block and fetches it from another instance of Apsis.
## Usage
```
Usage: apsisd [OPTIONS]

Options:
  -v, --verbose...           Increase logging verbosity
  -q, --quiet...             Decrease logging verbosity
  -b, --bind <BIND>          IP address and port to bind to
  -a, --auth <AUTH>          API authorization token
  -d, --database <DATABASE>  Path to Rocksdb database file
  -o, --opentelemetry        Enable Opentelemetry
  -h, --help                 Print help
  -V, --version              Print version
```
## License

[<img src="https://www.gnu.org/graphics/agplv3-with-text-162x68.png" alt="AGPLv3" >](https://www.gnu.org/licenses/agpl-3.0.html)

Apsis uses the excellent [eris-rs](https://github.com/mguentner/eris-rs) library from Maximilian Güntner to provide the encoding/decoding.
