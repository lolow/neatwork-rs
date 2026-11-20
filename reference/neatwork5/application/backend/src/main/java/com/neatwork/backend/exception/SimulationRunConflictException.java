package com.neatwork.backend.exception;


// Raised when an operation conflicts with the current lifecycle state of a simulation.
public class SimulationRunConflictException extends RuntimeException {

    public SimulationRunConflictException(String message) {
        super(message);
    }
}