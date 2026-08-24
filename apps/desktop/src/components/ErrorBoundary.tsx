import { Component, type ErrorInfo, type ReactNode } from "react";
import { formatBoundaryError, type FormattedBoundaryError } from "../errorBoundaryFormatting";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  caught: FormattedBoundaryError | null;
}

export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { caught: null };

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return { caught: formatBoundaryError(error, {}) };
  }

  componentDidCatch(error: unknown, errorInfo: ErrorInfo): void {
    const formatted = formatBoundaryError(error, errorInfo);
    console.error("[ErrorBoundary] caught render exception", error, errorInfo);
    this.setState({ caught: formatted });
  }

  render(): ReactNode {
    const { caught } = this.state;
    if (!caught) return this.props.children;

    return (
      <div className="error-boundary">
        <div className="error-boundary-card">
          <h1 className="error-boundary-title">Something went wrong</h1>
          <p className="error-boundary-subtitle">
            {caught.name}: {caught.message}
          </p>
          {caught.stack && <pre className="error-boundary-stack">{caught.stack}</pre>}
          {caught.componentStack && (
            <pre className="error-boundary-stack">{caught.componentStack}</pre>
          )}
          <button
            type="button"
            className="error-boundary-reload"
            onClick={() => window.location.reload()}
          >
            Reload
          </button>
        </div>
      </div>
    );
  }
}
