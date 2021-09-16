# steamgiftsbot-rust

```
Usage: steamgiftsbot [-f <cookie-file>] [-c <cookie>] [-d] [-t <filter-time>] [-s <sort-by>] [--reverse]

http://steamgifts.com bot written in Rust!
When no arguments supplied then a cookie will be read from `cookie.txt`

Options:
  -f, --cookie-file set a path to a cookie file
  -c, --cookie      cookie value, string after 'PHPSESSID=', automatically saves
                    to file
  -d, --daemon      daemonize
  -t, --filter-time filters giveaways that ends in X or earlier
  -s, --sort-by     sorting strategy allowed values are: [chance, price]
  --reverse         reverse sorting
  --help            display usage information

```
