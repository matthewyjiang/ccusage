# Maintainer: ccusage contributors <https://github.com/ccusage/ccusage>

pkgname=ccusage
_upstream_version=20.0.17
pkgver=${_upstream_version}.r1477.g400d383
pkgrel=1
pkgdesc='Analyze coding agent token usage and costs from local data'
arch=('x86_64' 'aarch64')
url='https://github.com/ccusage/ccusage'
license=('MIT')
depends=('gcc-libs' 'glibc')
makedepends=('cargo' 'git')
options=('!lto')
_litellm_rev=49ca04d8c3ddea336237ce6f3082dbc26d19e944
source=(
  "${pkgname}::git+file://${startdir}"
  "litellm-pricing-${_litellm_rev}.json::https://raw.githubusercontent.com/BerriAI/litellm/${_litellm_rev}/model_prices_and_context_window.json"
)
sha256sums=(
  'SKIP'
  'ae4532ba0c5da03ed694f37fffa050a65e0e250b816dcdb475bee0b7b7b1aa97'
)

pkgver() {
  cd "${pkgname}"
  local base_version
  base_version=$(sed -n 's/.*"version": "\([^"]*\)".*/\1/p' package.json | head -n 1)
  printf '%s.r%s.g%s' \
    "${base_version}" \
    "$(git rev-list --count HEAD)" \
    "$(git rev-parse --short=7 HEAD)"
}

prepare() {
  cd "${pkgname}"
  cargo fetch \
    --locked \
    --manifest-path rust/Cargo.toml \
    --target "${CARCH}-unknown-linux-gnu"
}

build() {
  cd "${pkgname}"
  export CCUSAGE_PRICING_JSON_PATH="${srcdir}/litellm-pricing-${_litellm_rev}.json"
  export CCUSAGE_VERSION="${_upstream_version}"
  cargo build \
    --frozen \
    --release \
    --bin ccusage \
    --manifest-path rust/Cargo.toml
}

check() {
  cd "${pkgname}"
  export CCUSAGE_PRICING_JSON_PATH="${srcdir}/litellm-pricing-${_litellm_rev}.json"
  export CCUSAGE_VERSION="${_upstream_version}"
  cargo test \
    --frozen \
    --workspace \
    --manifest-path rust/Cargo.toml
}

package() {
  cd "${pkgname}"
  install -Dm755 rust/target/release/ccusage "${pkgdir}/usr/bin/ccusage"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
