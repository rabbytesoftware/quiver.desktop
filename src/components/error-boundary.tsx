import { Component, type ErrorInfo, type JSX, type ReactNode } from 'react';

import { Button } from '@/components/ui/button';

import { useTranslation } from '@/lib/i18n';

interface CrashFallbackProps {
	onRetry: () => void;
}

export function CrashFallback({ onRetry }: CrashFallbackProps): JSX.Element {
	const { t } = useTranslation();
	return (
		<div className="flex h-full min-h-40 flex-col items-center justify-center gap-3 p-6 text-center" role="alert">
			<p className="text-sm font-medium">{t('app.crash.title')}</p>
			<p className="text-xs text-muted-foreground">{t('app.crash.body')}</p>
			<Button onClick={onRetry} size="sm" variant="outline">
				{t('app.crash.retry')}
			</Button>
		</div>
	);
}

interface ErrorBoundaryProps {
	children: ReactNode;
}

interface ErrorBoundaryState {
	failed: boolean;
}

/** Keeps one bad render from unmounting the whole app. */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
	state: ErrorBoundaryState = { failed: false };

	static getDerivedStateFromError(): ErrorBoundaryState {
		return { failed: true };
	}

	componentDidCatch(error: Error, info: ErrorInfo): void {
		console.error('render failed', error, info.componentStack);
	}

	private retry = (): void => {
		this.setState({ failed: false });
	};

	render(): ReactNode {
		return this.state.failed ? <CrashFallback onRetry={this.retry} /> : this.props.children;
	}
}
