import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

export function Header1({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return (
		<h1 className={cn("scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl", className)}>
			{children}
		</h1>
	)
}

export function Header2({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return (
		<h2 className={cn("scroll-m-20 border-b border-dashed pb-2 mb-2 text-3xl font-semibold tracking-tight first:mt-0", className)}>
			{children}
		</h2>
	)
}

export function Header3({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return (
		<h3 className={cn("scroll-m-20 text-2xl font-semibold tracking-tight", className)}>
			{children}
		</h3>
	)
}

export function Header4({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return (
		<h4 className={cn("scroll-m-20 text-xl font-semibold tracking-tight", className)}>
			{children}
		</h4>
	)
}

export function Paragraph({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return (
		<div className={cn("leading-7 not-first:mt-6", className)}>
			{children}
		</div>
	)
}

export function Span({
	children,
	className,
}: {
	children?: ReactNode
	className?: string
}) {
	return <span className={cn("", className)}>{children}</span>
}
