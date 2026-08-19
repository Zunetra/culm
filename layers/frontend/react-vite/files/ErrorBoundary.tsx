import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

// A class rather than a hook: React exposes error boundaries only through
// componentDidCatch, and there is still no function-component equivalent.
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Logged rather than swallowed. Without a report the boundary turns a crash
    // into a blank panel and nothing says why.
    console.error("unhandled error", error, info.componentStack);
  }

  render(): ReactNode {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }

    return (
      <section>
        <h1>Something went wrong</h1>
        <p>{import.meta.env.DEV ? error.message : "The page failed to load."}</p>
        <button type="button" onClick={() => this.setState({ error: null })}>
          Try again
        </button>
      </section>
    );
  }
}
