NAME=wasm-rust
BUILD_NAME=wasm_rust
TAG=v0.1
ENVOY_IMAGE=docker.io/istio/proxyv2:1.7.3

bootstrap-dev:
	rustup update
	rustup target add wasm32-unknown-unknown
	curl -sL https://run.solo.io/wasme/install | sh

build:
	cargo build

build-wasm:
	cargo build --target wasm32-unknown-unknown --release
	wasme build precompiled target/wasm32-unknown-unknown/release/$(BUILD_NAME).wasm --tag $(NAME):$(TAG)

deploy-envoy:
	wasme deploy envoy $(NAME):$(TAG) --envoy-image=$(ENVOY_IMAGE) --bootstrap=envoy-bootstrap.yaml --envoy-run-args="--log-level debug"

check-envoy:
	docker run --entrypoint "/usr/local/bin/envoy" $(ENVOY_IMAGE) --version
