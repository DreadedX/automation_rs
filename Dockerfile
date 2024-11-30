FROM gcr.io/distroless/cc-debian12:nonroot

ENV AUTOMATION_CONFIG=/app/config.lua
COPY ./config /app/config

COPY ./automation /app/automation

CMD ["/app/automation"]
