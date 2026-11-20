import { useEffect } from "react";

type WaterIntroTransitionProps = {
    onComplete: () => void;
};

const TRANSITION_DURATION = 4200;

export function WaterIntroTransition({ onComplete }: WaterIntroTransitionProps) {
    useEffect(() => {
        const prefersReducedMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
        const timeout = window.setTimeout(onComplete, prefersReducedMotion ? 250 : TRANSITION_DURATION);
        return () => {
            window.clearTimeout(timeout);
        };
    }, [onComplete]);

    return (
        <main className="water-transition" aria-label="Opening NeatWork workspace">
            <div className="water-transition__scene" aria-hidden="true">
                {/* Ambient background */}
                <div className="water-transition__ambient" />

                {/* 3D water surface */}
                <div className="water-transition__water-plane">
                    <div className="water-transition__ripple water-transition__ripple--1" />
                    <div className="water-transition__ripple water-transition__ripple--2" />
                    <div className="water-transition__ripple water-transition__ripple--3" />
                </div>

                {/* Water drop */}
                <div className="water-transition__drop-wrapper">
                    <div className="water-transition__drop" />
                </div>

                {/* Network */}
                <div className="water-transition__network">
                    <span className="water-transition__node water-transition__node--1" />
                    <span className="water-transition__node water-transition__node--2" />
                    <span className="water-transition__node water-transition__node--3" />
                    <span className="water-transition__node water-transition__node--4" />
                    <span className="water-transition__node water-transition__node--5" />

                    <span className="water-transition__pipe water-transition__pipe--1" />
                    <span className="water-transition__pipe water-transition__pipe--2" />
                    <span className="water-transition__pipe water-transition__pipe--3" />
                    <span className="water-transition__pipe water-transition__pipe--4" />
                </div>

                {/* Light travelling inside network */}
                <div className="water-transition__flow">
                    <span className="water-transition__flow-dot water-transition__flow-dot--1" />
                    <span className="water-transition__flow-dot water-transition__flow-dot--2" />
                    <span className="water-transition__flow-dot water-transition__flow-dot--3" />
                </div>
            </div>

            <div className="water-transition__content">
                <div className="water-transition__logo">
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M12 2.75C12 2.75 5.75 10.05 5.75 15.25C5.75 18.7 8.55 21.5 12 21.5C15.45 21.5 18.25 18.7 18.25 15.25C18.25 10.05 12 2.75 12 2.75Z" fill="currentColor" />
                    </svg>
                </div>

                <h1>NeatWork</h1>

                <p>Preparing your water network workspace </p>
            </div>
        </main>
    );
}