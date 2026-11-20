import {
    FormEvent,
    useEffect,
    useRef,
    useState,
} from "react";

import { createPortal } from "react-dom";

type NewProjectModalProps = {
    onClose: () => void;
    onCreate: (name: string) => Promise<void>;
};

export function NewProjectModal({ onClose, onCreate, }: NewProjectModalProps) {
    const [name, setName] = useState("");
    const [creating, setCreating] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    useEffect(() => {
        const previousOverflow = document.body.style.overflow;
        document.body.style.overflow = "hidden";
        const frame = window.requestAnimationFrame(() => { inputRef.current?.focus(); });
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !creating) {
                onClose();
            }
        }

        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.cancelAnimationFrame(frame);
            document.body.style.overflow = previousOverflow;

            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [onClose, creating]);

    async function handleSubmit(event: FormEvent) {
        event.preventDefault();
        const projectName = name.trim();
        if (!projectName || creating) {
            return;
        }

        setCreating(true);
        setError(null);
        try {
            await onCreate(projectName);
        } catch (error) {
            console.error("Could not create project:", error);
            setError("The project could not be created.");
        } finally {
            setCreating(false);
        }
    }

    return createPortal(
        <div className="project-modal-backdrop" onMouseDown={(event) => {
            if (event.target === event.currentTarget && !creating) {
                onClose();
            }
        }}
        >
            <form
                className="project-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="new-project-title"
                onSubmit={handleSubmit}
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Project</span>
                        <h2 id="new-project-title">New Project</h2>
                    </div>

                    <button
                        type="button"
                        className="project-modal__close"
                        aria-label="Close"
                        disabled={creating}
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>

                <div className="project-modal__body">
                    <label className="project-modal__field">
                        <span>Project name</span>

                        <input
                            ref={inputRef}
                            type="text"
                            value={name}
                            maxLength={120}
                            placeholder="Example: Toy network"
                            disabled={creating}
                            onChange={(event) =>
                                setName(event.target.value)
                            }
                        />
                    </label>

                    <p className="project-modal__hint">Enter a name that will help you identify this water network project.</p>

                    {error && (
                        <div className="project-modal__error">{error}</div>
                    )}
                </div>

                <footer className="project-modal__footer">
                    <button
                        type="button"
                        className="project-modal__secondary"
                        disabled={creating}
                        onClick={onClose}
                    >
                        Cancel
                    </button>

                    <button
                        type="submit"
                        className="project-modal__primary"
                        disabled={
                            creating || !name.trim()
                        }
                    >
                        {creating
                            ? "Creating..."
                            : "Create project"}
                    </button>
                </footer>
            </form>
        </div>,
        document.body
    );
}