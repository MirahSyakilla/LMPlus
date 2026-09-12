# Supported targets
TARGET_IOS     := aarch64-apple-ios
TARGET_MACOS   := aarch64-apple-darwin
TARGET_LINUX   := x86_64-unknown-linux-gnu
TARGET_ANDROID := aarch64-linux-android
TARGET_WINDOWS := x86_64-pc-windows-msvc

.PHONY: all build build-release check clippy fmt \
        build-ios build-ios-release  \
        build-macos build-macos-release  \
        build-linux build-linux-release  \
        build-android build-android-release  \
        build-windows build-windows-release  \
        clean doc

all: build-ios-release build-macos-release build-linux-release build-android-release build-windows-release

build:
	cargo build

build-release:
	cargo build --release

check:
	cargo check

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

build-ios:
	cargo build --target $(TARGET_IOS)

build-ios-release:
	cargo build --target $(TARGET_IOS) --release

build-macos:
	cargo build --target $(TARGET_MACOS)

build-macos-release:
	cargo build --target $(TARGET_MACOS) --release

build-linux:
	cargo build --target $(TARGET_LINUX)

build-linux-release:
	cargo build --target $(TARGET_LINUX) --release

build-android:
	cargo build --target $(TARGET_ANDROID)

build-android-release:
	cargo build --target $(TARGET_ANDROID) --release

build-windows:
	cargo build --target $(TARGET_WINDOWS)

build-windows-release:
	cargo build --target $(TARGET_WINDOWS) --release

doc:
	cargo doc --no-deps --open

clean:
	cargo clean
