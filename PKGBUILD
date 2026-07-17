# Maintainer: ccusage contributors <https://github.com/ccusage/ccusage>

pkgname=ccusage
pkgver=20.0.17
pkgrel=1
pkgdesc='Analyze coding agent token usage and costs from local data'
arch=('x86_64' 'aarch64')
url='https://github.com/ccusage/ccusage'
license=('MIT')
depends=('gcc-libs' 'glibc')
makedepends=('cargo')
options=('!lto')
_litellm_rev=49ca04d8c3ddea336237ce6f3082dbc26d19e944
source=(
  "${pkgname}-${pkgver}.tar.gz::${url}/archive/refs/tags/v${pkgver}.tar.gz"
  "litellm-pricing-${_litellm_rev}.json::https://raw.githubusercontent.com/BerriAI/litellm/${_litellm_rev}/model_prices_and_context_window.json"
)
sha256sums=(
  '8e4d2693641dcbee649ee5c1d3c4b809d62a46f84c1e199c44995587ce45d20c'
  'ae4532ba0c5da03ed694f37fffa050a65e0e250b816dcdb475bee0b7b7b1aa97'
)

prepare() {
  cd "${pkgname}-${pkgver}"
  cargo fetch \
    --locked \
    --manifest-path rust/Cargo.toml \
    --target "${CARCH}-unknown-linux-gnu"
}

build() {
  cd "${pkgname}-${pkgver}"
  export CCUSAGE_PRICING_JSON_PATH="${srcdir}/litellm-pricing-${_litellm_rev}.json"
  export CCUSAGE_VERSION="${pkgver}"
  cargo build \
    --frozen \
    --release \
    --bin ccusage \
    --manifest-path rust/Cargo.toml
}

check() {
  cd "${pkgname}-${pkgver}"
  export CCUSAGE_PRICING_JSON_PATH="${srcdir}/litellm-pricing-${_litellm_rev}.json"
  export CCUSAGE_VERSION="${pkgver}"
  cargo test \
    --frozen \
    --workspace \
    --manifest-path rust/Cargo.toml
}

package() {
  cd "${pkgname}-${pkgver}"
  install -Dm755 rust/target/release/ccusage "${pkgdir}/usr/bin/ccusage"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
