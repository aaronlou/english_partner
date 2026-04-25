"""FastAPI entry point for English Partner AI service."""

from __future__ import annotations

from contextlib import asynccontextmanager

import structlog
from dotenv import load_dotenv
from fastapi import FastAPI

from ep_ai.api.routes import router

load_dotenv()
logger = structlog.get_logger()


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info("ai_service.starting")
    yield
    logger.info("ai_service.stopping")


app = FastAPI(title="English Partner AI", lifespan=lifespan)
app.include_router(router)

if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="127.0.0.1", port=8000)
