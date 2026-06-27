from __future__ import annotations


class NetRailError(Exception):
    """Typed error with stable API code (parity with Rust NetRailError)."""

    def __init__(self, code: str, message: str, *, status: int = 400) -> None:
        self.code = code
        self.message = message
        self.status = status
        super().__init__(message)

    def to_dict(self) -> dict[str, str | int]:
        return {"code": self.code, "detail": self.message, "status": self.status}