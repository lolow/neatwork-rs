import pytest


@pytest.fixture
def sample_client_rows():
    return [
        {
            "client": "C1",
            "borne": "P3",
            "proba": 0.4,
            "cible": 0.2,
        },
        {
            "client": "C2",
            "borne": "P4",
            "proba": 0.4,
            "cible": 0.2,
        },
    ]