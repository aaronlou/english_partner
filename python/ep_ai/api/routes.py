"""FastAPI HTTP endpoints for the English Partner AI service."""

from __future__ import annotations

from fastapi import APIRouter, File, Form, HTTPException, UploadFile
from pydantic import BaseModel

from ep_ai.providers import get_provider
from ep_ai.services.conversation import ConversationService, SessionNotFoundError
from ep_ai.services.scenario import list_scenarios

router = APIRouter()

# Shared service instance (provider resolved from LLM_PROVIDER env var).
_service = ConversationService(provider=get_provider())


# =============================================================================
# Pydantic Models
# =============================================================================


class StartResponse(BaseModel):
    session_id: str
    ai_text: str
    audio_base64: str
    scenario_name: str


class TurnResponse(BaseModel):
    transcript: str
    ai_text: str
    audio_base64: str


class HealthResponse(BaseModel):
    status: str
    provider: str
    provider_connected: bool


class ScenarioListResponse(BaseModel):
    scenarios: list[dict]


# =============================================================================
# Endpoints
# =============================================================================


@router.get("/health", response_model=HealthResponse)
async def health() -> HealthResponse:
    health_data = _service.health()
    return HealthResponse(
        status=health_data["status"],
        provider=_service.provider.name,
        provider_connected=health_data.get(f"{_service.provider.name}_connected", False),
    )


@router.get("/scenarios", response_model=ScenarioListResponse)
async def list_scenario_endpoints() -> ScenarioListResponse:
    return ScenarioListResponse(scenarios=list_scenarios())


@router.post("/conversation/start", response_model=StartResponse)
async def start_conversation(scenario_id: str = Form("restaurant")) -> StartResponse:
    """Initialize a new conversation scenario."""
    result = await _service.start(scenario_id)
    return StartResponse(**result)


@router.post("/conversation/turn", response_model=TurnResponse)
async def conversation_turn(
    session_id: str = Form(...),
    audio: UploadFile = File(...),
) -> TurnResponse:
    """Process one conversation turn: student audio in -> AI response out."""
    audio_bytes = await audio.read()
    try:
        result = await _service.turn(session_id, audio_bytes)
        return TurnResponse(**result)
    except SessionNotFoundError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc


@router.post("/conversation/respond", response_model=TurnResponse)
async def respond_text(
    session_id: str = Form(...),
    student_text: str = Form(...),
) -> TurnResponse:
    """Text-only fallback response (no audio input)."""
    try:
        result = await _service.respond_text(session_id, student_text)
        return TurnResponse(**result)
    except SessionNotFoundError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
