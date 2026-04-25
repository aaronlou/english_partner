"""Provider registry — dynamic LLM backend switching."""

from __future__ import annotations

import os
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .base import LLMProvider


class ProviderRegistry:
    """Central registry for LLM provider implementations.

    Usage:
        registry = ProviderRegistry()
        registry.register("mimo", MiMoProvider)
        registry.register("openai", OpenAIProvider)

        provider = registry.get("mimo")  # or os.environ["LLM_PROVIDER"]
    """

    def __init__(self) -> None:
        self._providers: dict[str, type] = {}

    def register(self, name: str, provider_cls: type) -> None:
        self._providers[name] = provider_cls

    def get(self, name: str) -> LLMProvider:
        if name not in self._providers:
            raise KeyError(
                f"Unknown provider '{name}'. "
                f"Available: {list(self._providers.keys())}"
            )
        return self._providers[name]()

    def list(self) -> list[str]:
        return list(self._providers.keys())


# Global singleton registry.
_registry = ProviderRegistry()


def register_provider(name: str, provider_cls: type) -> None:
    _registry.register(name, provider_cls)


def get_provider(name: str | None = None) -> LLMProvider:
    if name is None:
        name = os.environ.get("LLM_PROVIDER", "mimo")
    return _registry.get(name)


def list_providers() -> list[str]:
    return _registry.list()


# Re-export base types for convenience.
from .base import LLMProvider  # noqa: E402
from .base import LLMProviderError as LLMProviderError  # noqa: E402

# Auto-register built-in providers.
from .mimo import MiMoProvider  # noqa: E402
from .openai import OpenAIProvider  # noqa: E402

register_provider("mimo", MiMoProvider)
register_provider("openai", OpenAIProvider)
