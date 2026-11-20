type StartScreenProps = {
    onStart: () => void;
};

export function StartScreen({ onStart }: StartScreenProps) {
    return (
        <main className="start-screen">
            <div className="start-screen__background" aria-hidden="true">
                <div className="start-screen__glow start-screen__glow--one" />
                <div className="start-screen__glow start-screen__glow--two" />
            </div>

            <header className="start-screen__brand">
                <div className="start-screen__brand-mark" aria-hidden="true">
                    <svg viewBox="0 0 24 24" focusable="false">
                        <path d="M12 2.75C12 2.75 5.75 10.05 5.75 15.25C5.75 18.7 8.55 21.5 12 21.5C15.45 21.5 18.25 18.7 18.25 15.25C18.25 10.05 12 2.75 12 2.75Z" fill="currentColor" />
                    </svg>
                </div>

                <span className="start-screen__brand-name">NeatWork</span>
            </header>

            <section className="start-screen__hero">
                <p className="start-screen__eyebrow">Water Network Design & Simulation</p>

                <h1 className="start-screen__title">NeatWork</h1>

                <p className="start-screen__description">Design, simulate and analyse gravity-driven water distribution networks.</p>

                <button type="button" className="start-screen__button" onClick={onStart}>
                    <span>Start</span>

                    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                        <path
                            d="M5 12h14M13 6l6 6-6 6"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="2"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                        />
                    </svg>
                </button>
            </section>

            <footer className="start-screen__footer">
                <span>Gravity-driven water distribution systems Design by<b> APLV</b></span>
            </footer>
        </main>
    );
}