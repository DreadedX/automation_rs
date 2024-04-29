FROM gcr.io/distroless/cc-debian12:nonroot

ENV AUTOMATION_CONFIG=/app/config.lua
COPY ./config.lua /app/config.lua

COPY ./build/automation /app/automation

CMD ["/app/automation"]
