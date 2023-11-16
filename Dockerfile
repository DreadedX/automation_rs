FROM gcr.io/distroless/cc-debian12:nonroot

ENV AUTOMATION_CONFIG=/app/config.yml
COPY ./config/config.yml /app/config.yml

COPY ./build/automation /app/automation

CMD ["/app/automation"]
