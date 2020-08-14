# Macchiato DNS
A simple DNS proxy server written in rust. Macchiato DNS contains some powerful features that can be used to secure your
communications.

## DoH
Macchiato DNS uses DoH to communicate securely with trusted DNS servers. This means that running Macchiato DNS locally
will pervents your ISP from snooping on your DNS traffic.

## Blocklist
Macchiato also uses [energized block lists](https://github.com/EnergizedProtection/block) to prevent you computer from
communicating with known ad and malware servers. This will improve web page load performance and reduce the chance of
downloading a malware.

## Caching
Macchiato DNS caches DNS responses according to their respective TTLs. This should accelerate web browsing and reduce
bandwidth usage.

## Flags
Loading the block list will take a considerable amount of memory on small devices (think raspberry-pi). For that reason,
if you are willing to cut on RAM usage but reduce the performance by some percents, the flag `--small` can be used.

By default, the `blu` block list is used. This list is a good balance between size and blocking the right domains.
However, the `ultimate` list can be used if you wish to block even more domains. Use the flag `--filter-list` to
customize that behavior. If you don't want to block any domain, you can pass `none` to this flag.

## Installation
There is currently no prebuilt binaries. However, the project is easy to compile with an up to date rust toolchain.
```bash
$ cargo build --release
```

### Systemd
If you want to run Macchiato DNS as a service on Linux, you can manually move the required files and enable the service.

```bash
$ mv target/release/dns /usr/bin/macchiato-dns
$ chown sudo:sudo /usr/bin/macchiato-dns
$ cp blu.txt /var/lib/macchiato-dns/
$ chown sudo:sudo /var/lib/macchiato-dns/blu.txt
$ cp macchiato.service /usr/lib/systemd/system/
$ systemctl enable macchiato.service --now
```

### resolv.conf
Finally, you will need to setup your OS to use your local computer as its DNS resolver. Using your favorite text editor,
comment out with `#` every line starting with `nameserver` in `/etc/resolv.conf` and add `nameserver 127.0.0.1`.

## Contributing
If you think there's a missing feature, feel free to open a pull request. I will be more than happy to help you
contribute.

I'm not planning to add any new features by myself for now (like auto updating block lists). The project has currently
enough features for my personal use.

# License
This software is distributed under Apache-2.0.

It is using [lists from energized](https://github.com/EnergizedProtection/block) which are distributed under MIT.

It is also using [Font Awesome Free](https://fontawesome.com/license).
