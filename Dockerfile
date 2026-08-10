# Compatibility image (Python FastAPI). Prefer Dockerfile.rust + netrail-api for production Docker.
FROM python:3.12-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY netrail ./netrail
COPY assets ./assets

ENV NETRAIL_DB_PATH=/app/data/netrail.db \
    NETRAIL_AUTO_OPEN=false \
    NETRAIL_HISTORY_ENCRYPT=true

RUN mkdir -p /app/data /app/.config /app/.local/share/netrail \
    && useradd --system --home /app --no-create-home appuser \
    && chown -R appuser:appuser /app/data /app/.config /app/.local/share

ENV HOME=/app

USER appuser

EXPOSE 7421

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://127.0.0.1:7421/api/health')"

CMD ["python", "-m", "netrail"]