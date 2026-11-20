package com.neatwork.backend.service;

import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;

import org.springframework.boot.context.event.ApplicationReadyEvent;
import org.springframework.context.event.EventListener;
import org.springframework.stereotype.Service;


@Service
@RequiredArgsConstructor
@Slf4j
public class SimulationRecoveryService {
    private final SimulationRunService simulationRunService;

    // Finalize jobs that were interrupted by a previous backend process termination.
    @EventListener(ApplicationReadyEvent.class)
    public void recoverInterruptedJobs() {
        var count = simulationRunService.failInterruptedExecutions();

        if (count > 0) {
            log.warn("{} simulation execution(s) were marked as failed after backend restart.", count);
        }
    }
}