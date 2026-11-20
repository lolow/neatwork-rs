package com.neatwork.backend.exception;

public class DomainIntegrityException extends RuntimeException {

    public DomainIntegrityException(String message) {
        super(message);
    }
}
