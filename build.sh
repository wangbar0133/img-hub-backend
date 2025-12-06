#!/bin/bash
set -e

# 默认配置
DOCKER_REPO="${DOCKER_REPO:-wangbar01334/img-hub-backend}"
TAG="${TAG:-latest}"
PLATFORM="${PLATFORM:-linux/amd64}"
PUSH="${PUSH:-true}"

echo "🔧 Building Rust application..."

# 检查是否在 macOS M1/M2 上
if [[ "$(uname)" == "Darwin" && "$(uname -m)" == "arm64" ]]; then
    echo "🍎 Detected Apple Silicon - cross compiling for Linux x86_64 with musl"

    # 确保目标已安装
    rustup target add x86_64-unknown-linux-musl

    # 使用musl进行静态链接编译
    echo "Building with musl for static linking..."
    cargo build --release --target x86_64-unknown-linux-musl

    # 复制到标准位置
    mkdir -p target/release/
    cp target/x86_64-unknown-linux-musl/release/img-hub-backend target/release/
else
    echo "💻 Building natively..."
    cargo build --release
fi

echo "🐳 Building Docker image..."

if [[ "$PUSH" == "true" ]]; then
    echo "Building and pushing to $DOCKER_REPO:$TAG for platform $PLATFORM"
    docker buildx build --platform $PLATFORM \
        -t $DOCKER_REPO:$TAG \
        --push .
else
    echo "Building locally as $DOCKER_REPO:$TAG for platform $PLATFORM"
    docker buildx build --platform $PLATFORM \
        -t $DOCKER_REPO:$TAG \
        --load .
fi

echo "✅ Build completed successfully!"

if [[ "$PUSH" == "true" ]]; then
    echo "🚀 Image pushed to Docker Hub: $DOCKER_REPO:$TAG"
    echo "Pull with: docker pull $DOCKER_REPO:$TAG"
else
    echo "🏃 Run locally with: docker run -p 8000:8000 -v \$(pwd)/static:/app/static $DOCKER_REPO:$TAG"
fi