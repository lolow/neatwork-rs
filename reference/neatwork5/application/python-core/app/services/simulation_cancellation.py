"""Thread-safe cancellation registry for running simulations."""

from threading import Event, Lock


_events: dict[str, Event] = {}
_lock = Lock()


def register_execution(execution_id: str) -> None:
    """Register one simulation execution."""
    with _lock:
        _events.setdefault(execution_id, Event())

def request_cancel(execution_id: str) -> bool:
    """Mark one execution as cancellation-requested."""
    with _lock:
        event = _events.setdefault(execution_id, Event())
        event.set()
    return True

def is_cancel_requested(execution_id: str) -> bool:
    """Return whether cancellation was requested."""
    with _lock:
        event = _events.get(execution_id)
        return bool(event and event.is_set())

def clear_execution(execution_id: str) -> None:
    """Remove one completed execution from the registry."""
    with _lock:
        _events.pop(execution_id, None)