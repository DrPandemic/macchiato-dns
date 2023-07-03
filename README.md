# Macchiato DNS
A simple DNS proxy server written in rust. Macchiato DNS contains some powerful features that can be used to secure your
communications.

## DoH
Macchiato DNS uses DoH to communicate securely with trusted DNS servers. This means that running Macchiato DNS locally
will pervents your ISP from snooping on your DNS traffic.

### Multiple DoH servers
Macchiato is trying to use multiple DNS servers and will will the one(s) giving you the best performance.

## Blocklist
Macchiato also uses [energized block lists](https://github.com/EnergizedProtection/block) to prevent you computer from
communicating with known ad and malware servers. This will improve web page load performance and reduce the chance of
downloading a malware.

## Caching
Macchiato DNS caches DNS responses according to their respective TTLs. This should accelerate web browsing and reduce
bandwidth usage.

## Simple Web UI
The web UI lets you track what's happening on your network and update allowed domains.

![macchiato](https://user-images.githubusercontent.com/3250155/90339707-81a4d680-dfc0-11ea-9b59-c62dcadd7ba8.jpg)

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

### Run on an external device
I personally run my Macchiatod-dns server on a Raspberry Pi 3b somewhere on my network and configured my router to use it has its DNS server. That's the command I use to launch it in docker.
```
$ docker run -d --rm -v /path/to/ssl/certs:/app/ssl -p 8080:80 -p 53:53/udp --name macchiato-dns macchiato-dns -e --allowed foo.bar baz.foo
```

## TODO

- [ ] Shouldn't ignore queries that failed to parse.
- [ ] DNSSEC.
  - [ ] Fragmented datagrams.
    - Do we actually need it since we send the requests over HTTPS?
    - [ ] A security-aware resolver MUST support a message size of at least 1220 octets, SHOULD support a message size of 4000 octets.
  - [ ] EDNS.
- [ ] ODoH. https://tools.ietf.org/html/draft-pauly-dprive-oblivious-doh-02#section-5
  - https://blog.cloudflare.com/oblivious-dns/
  - https://github.com/cloudflare/odoh-client-rs/blob/master/src/dns_utils.rs
  - [ ] HPKE. https://tools.ietf.org/html/draft-irtf-cfrg-hpke-07
    - https://docs.rs/hpke/0.3.1/hpke/index.html
- [ ] TCP fallbacks.
- [ ] IPv6.

## Cross compiling
https://github.com/briansmith/webpki/issues/54 prevents us from using rustls which would make cross compilation much easier.
OpenSSL makes this app more complicated to compile. This command should let you cross compile to raspberry pi.

``` bash
$ MACHINE=armv7 ARCH=arm CC=arm-linux-gnueabihf-gcc cargo build --release --target=armv7-unknown-linux-gnueabihf
```

### Building the ARM docker image

```bash
$ docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
$ docker build -f Dockerfile-compile-arm -t drpandemic/macchiato-dns .
$ docker push drpandemic/macchiato-dns
```

### Building the x86 docker image

```bash
$ docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
$ docker build -f Dockerfile-compile -t drpandemic/macchiato-dns .
$ docker push drpandemic/macchiato-dns
```

## Contributing
If you think there's a missing feature, feel free to open a pull request. I will be more than happy to help you
contribute.

I'm not planning to add any new features by myself for now. The project has currently enough features for my personal
use.

# License
This software is distributed under Apache-2.0.

It is using [lists from energized](https://github.com/EnergizedProtection/block) which are distributed under MIT.

It is also using [Font Awesome Free](https://fontawesome.com/license).
