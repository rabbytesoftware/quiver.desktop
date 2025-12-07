import Preact from 'preact';

export function Header1({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: Preact.ComponentChildren;
	className?: string;
	useSecondaryFont?: boolean;
}) {
	const fontClass = useSecondaryFont ? '' : '';
	return (
		<h1 className={`scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl ${fontClass} ${className}`}>
			{children}
		</h1>
	);
}

export function Header2({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: Preact.ComponentChildren;
	className?: string;
	useSecondaryFont?: boolean;
}) {
	const fontClass = useSecondaryFont ? '' : '';
	return (
		<h2
			className={`${fontClass} scroll-m-20 border-b border-dashed pb-2 mb-2 text-3xl font-semibold tracking-tight first:mt-0 ${className}`}
		>
			{children}
		</h2>
	);
}

export function Header3({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: Preact.ComponentChildren;
	className?: string;
	useSecondaryFont?: boolean;
}) {
	const fontClass = useSecondaryFont ? '' : '';
	return (
		<h3 className={`scroll-m-20 text-2xl font-semibold tracking-tight ${fontClass} ${className}`}>{children}</h3>
	);
}

export function Header4({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: Preact.ComponentChildren;
	className?: string;
	useSecondaryFont?: boolean;
}) {
	const fontClass = useSecondaryFont ? '' : '';
	return <h4 className={`${fontClass} scroll-m-20 text-xl font-semibold tracking-tight ${className}`}>{children}</h4>;
}

export function Paragraph({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: Preact.ComponentChildren;
	className?: string;
	useSecondaryFont?: boolean;
}) {
	const fontClass = useSecondaryFont ? '' : '';
	return <div className={`leading-7 not-first:mt-6 ${fontClass} ${className}`}>{children}</div>;
}

export function Span({
	children,
	className,
	useSecondaryFont = false,
}: {
	children?: Preact.ComponentChildren;
	className?: string;
	useSecondaryFont?: boolean;
}) {
	const fontClass = useSecondaryFont ? '' : '';
	return <span className={`${fontClass} ${className}`}>{children}</span>;
}
