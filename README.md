# Apsis
Apsis is a prototype server using the _Encoding for Robust Immutable Storage_ [spec](https://eris.codeberg.page/spec/).
## How it works
Files or JSON data encoded using ERIS are split up into encrypted blocks and stored in the database. Individual blocks are also advertised via the Bittorrent Mainline DHT. When Apsis is missing a block, it performs a lookup on the DHT for the missing block and fetches it from another instance of Apsis.

Apsis exposes a simple API based on [RFC2169](https://datatracker.ietf.org/doc/html/rfc2169), extended to support file uploads. An HTTP `POST` to `/uri-res/R2N` will upload the data (such as a JSON string or arbitrary file) and return an ERIS URN (a matching token in the `Authorization` header is required to upload). An HTTP `GET` to `/uri-res/N2R?<ERIS URN>` will return the data.

A simple client, `apsisctl`, is provided for convenience but it's almost equally simple to use `curl`.

**NOTE:** For block discovery, this has the same network limitations as seeding a file with Bittorrent, namely the `apsisd` instance serving a block needs to have its port exposed to the internet.
## Usage

Server:
```
Usage: apsisd [OPTIONS]

Options:
  -v, --verbose...           Increase logging verbosity
  -q, --quiet...             Decrease logging verbosity
  -b, --bind <BIND>          IP address and port to bind to
  -p, --port <PORT>          Port to advertise (otherwise uses bind port)
  -a, --auth <AUTH>          API authorization token
  -d, --database <DATABASE>  Path to Rocksdb database file
  -o, --opentelemetry        Enable Opentelemetry
  -h, --help                 Print help
  -V, --version              Print version
```

Client:
```
Usage: apsisctl [OPTIONS] --connect <CONNECT> <COMMAND>

Commands:
  upload    Upload JSON or file data
  download  Download JSON or file data
  help      Print this message or the help of the given subcommand(s)

Options:
  -c, --connect <CONNECT>  IP address and port to connect to
  -v, --verbose...         Increase logging verbosity
  -q, --quiet...           Decrease logging verbosity
  -h, --help               Print help
  -V, --version            Print version
```

## License

[<img src="https://www.gnu.org/graphics/agplv3-with-text-162x68.png" alt="AGPLv3" >](https://www.gnu.org/licenses/agpl-3.0.html)

Apsis uses the excellent [eris-rs](https://github.com/mguentner/eris-rs) library from Maximilian Güntner to provide the encoding/decoding.
