"""FastAPI entry point for AI orchestration service."""

import os
from contextlib import asynccontextmanager

import structlog
from fastapi import FastAPI

logger = structlog.get_logger()


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info("ai_service.starting")
    yield
    logger.info("ai_service.stopping")


app = FastAPI(title="English Partner AI", lifespan=lifespan)


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/conversation/start")
async def start_conversation(scenario_id: str) -> dict:
    """Initialize a new conversation scenario."""
    return {"scenario_id": scenario_id, "message": "Hello! Let's practice English."}


@app.post("/conversation/respond")
async def respond(student_text: str, scenario_id: str) -> dict:
    """Generate AI response to student's utterance."""
    # TODO: integrate LLM
    return {
        "ai_text": f"That's interesting! Tell me more about that.",
        "corrections": [],
        "suggestions": [],
    }


@app.post("/assessment/pronunciation")
async def assess_pronunciation(student_text: str, reference_text: str) -> dict:
    """Assess pronunciation quality.

    TODO: Integrate Azure Pronunciation Assessment or SpeechSuper.
    Fallback: use text similarity + confidence scores.
    """
    return {"score": 0.85, "weak_phonemes": [], "feedback": "Good job!"}


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8000)
