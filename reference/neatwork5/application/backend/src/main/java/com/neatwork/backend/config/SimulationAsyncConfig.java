package com.neatwork.backend.config;

import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.scheduling.concurrent.ThreadPoolTaskExecutor;

import java.util.concurrent.Executor;

/**
 * Run independent simulations concurrently.
 * Four fixed workers allow multiple jobs to execute without forcing simulations into one sequential queue.
 */
@Configuration
public class SimulationAsyncConfig {

    @Bean(name = "simulationTaskExecutor")
    public Executor simulationTaskExecutor() {
        var executor = new ThreadPoolTaskExecutor();
        executor.setCorePoolSize(4);
        executor.setMaxPoolSize(4);
        executor.setQueueCapacity(32);

        executor.setThreadNamePrefix("neatwork-simulation-");
        executor.initialize();

        return executor;
    }
}