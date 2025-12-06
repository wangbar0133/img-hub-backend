# Rust 后端 Dockerfile - 仅运行时镜像

# 运行阶段 - 使用更小的基础镜像（静态链接）
FROM alpine:3.18

# 安装运行时依赖
RUN apk --no-cache add \
    ca-certificates \
    curl

# 创建应用用户
RUN adduser -D -s /bin/false appuser

# 创建应用目录
WORKDIR /app

# 复制本地编译好的二进制文件
COPY target/release/img-hub-backend /app/

# 创建静态文件目录和日志目录
RUN mkdir -p /app/static /app/logs && chown -R appuser:appuser /app

# 声明数据卷
VOLUME ["/app/static", "/app/logs"]

# 切换到应用用户
USER appuser

# 暴露端口
EXPOSE 8000

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8000/ || exit 1

# 启动命令 - 输出日志到文件和控制台
CMD ["sh", "-c", "./img-hub-backend 2>&1 | tee /app/logs/app.log"]