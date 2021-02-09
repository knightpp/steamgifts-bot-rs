# Maintainer: Daniel <knightpp@pm.me>
pkgname=steamgiftsbot
pkgver=0.3.1.r0.g2e30200
pkgrel=1
pkgdesc="Automatically enter giveaways for steamgifts.com"
license=("MIT|Apache-2.0")
arch=("x86_64")
makedepends=("cargo")

pkgver() {
    (git describe --long --tags || echo "$pkgver") | sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
}

build() {
    return 0
}

package() {
    cd ..
    usrdir="$pkgdir/usr"
    mkdir -p $usrdir
    cargo install --no-track --path . --root "$usrdir"
}

