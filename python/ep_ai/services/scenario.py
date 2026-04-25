"""Scenario loading and management."""

from __future__ import annotations

from ep_ai.prompts.scenarios import SCENARIOS


def get_scenario(scenario_id: str) -> dict:
    """Return a scenario definition by ID, falling back to 'restaurant'."""
    return SCENARIOS.get(scenario_id, SCENARIOS["restaurant"])


def list_scenarios() -> list[dict]:
    """Return all scenarios as a list."""
    return [
        {"id": key, "name": value["name"], "description": value.get("description", "")}
        for key, value in SCENARIOS.items()
    ]
