import { Component, type ErrorInfo, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";
import { formatBoundaryError, type FormattedBoundaryError } from "../errorBoundaryFormatting";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  caught: FormattedBoundaryError | null;
  copied: boolean;
}

function boundaryErrorText(caught: FormattedBoundaryError): string {
  return [
    `${caught.name}: ${caught.message}`,
    caught.stack,
    caught.componentStack,
  ]
    .filter(Boolean)
    .join("\n\n");
}

export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { caught: null, copied: false };
  private copiedResetTimer: ReturnType<typeof setTimeout> | null = null;

  static getDerivedStateFromError(error: unknown): Pick<ErrorBoundaryState, "caught"> {
    return { caught: formatBoundaryError(error, {}) };
  }

  componentDidCatch(error: unknown, errorInfo: ErrorInfo): void {
    const formatted = formatBoundaryError(error, errorInfo);
    console.error("[ErrorBoundary] caught render exception", error, errorInfo);
    this.setState({ caught: formatted });
  }

  componentWillUnmount(): void {
    if (this.copiedResetTimer) clearTimeout(this.copiedResetTimer);
  }

  private handleCopy = async (): Promise<void> => {
    const { caught } = this.state;
    if (!caught) return;
    try {
      await navigator.clipboard.writeText(boundaryErrorText(caught));
    } catch (err) {
      console.error("[ErrorBoundary] failed to copy error details", err);
      return;
    }
    if (this.copiedResetTimer) clearTimeout(this.copiedResetTimer);
    this.setState({ copied: true });
    this.copiedResetTimer = setTimeout(() => this.setState({ copied: false }), 1500);
  };

  render(): ReactNode {
    const { caught, copied } = this.state;
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
          <div className="error-boundary-actions">
            <button type="button" className="error-boundary-copy" onClick={this.handleCopy}>
              {copied ? <Check size={14} /> : <Copy size={14} />}
              {copied ? "Copied" : "Copy to clipboard"}
            </button>
            <button
              type="button"
              className="error-boundary-reload"
              onClick={() => window.location.reload()}
            >
              Reload
            </button>
          </div>
        </div>
      </div>
    );
  }
}
