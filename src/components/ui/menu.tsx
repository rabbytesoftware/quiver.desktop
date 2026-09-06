'use client';

import type React from 'react';

import { Menu as MenuPrimitive } from '@base-ui/react/menu';
import { Separator as SeparatorPrimitive } from '@base-ui/react/separator';

import { cn } from '@/lib/cn';

export const Menu: typeof MenuPrimitive.Root = MenuPrimitive.Root;

export function MenuTrigger(props: MenuPrimitive.Trigger.Props): React.ReactElement {
	return <MenuPrimitive.Trigger data-slot="menu-trigger" {...props} />;
}

export function MenuPopup({
	className,
	align = 'end',
	sideOffset = 4,
	side = 'bottom',
	portalProps,
	...props
}: MenuPrimitive.Popup.Props & {
	align?: MenuPrimitive.Positioner.Props['align'];
	side?: MenuPrimitive.Positioner.Props['side'];
	sideOffset?: MenuPrimitive.Positioner.Props['sideOffset'];
	portalProps?: MenuPrimitive.Portal.Props;
}): React.ReactElement {
	return (
		<MenuPrimitive.Portal {...portalProps}>
			<MenuPrimitive.Positioner align={align} className="z-50 outline-none" side={side} sideOffset={sideOffset}>
				<MenuPrimitive.Popup
					className={cn(
						'min-w-40 origin-(--transform-origin) rounded-lg border bg-popover p-1 text-popover-foreground not-dark:bg-clip-padding shadow-lg/5 outline-none',
						'before:pointer-events-none before:absolute before:inset-0 before:rounded-[calc(var(--radius-lg)-1px)] before:shadow-[0_1px_--theme(--color-black/4%)] dark:before:shadow-[0_-1px_--theme(--color-white/6%)]',
						'data-ending-style:scale-98 data-ending-style:opacity-0 data-starting-style:scale-98 data-starting-style:opacity-0',
						className
					)}
					data-slot="menu-popup"
					{...props}
				/>
			</MenuPrimitive.Positioner>
		</MenuPrimitive.Portal>
	);
}

export function MenuItem({
	className,
	variant = 'default',
	...props
}: MenuPrimitive.Item.Props & { variant?: 'default' | 'destructive' }): React.ReactElement {
	return (
		<MenuPrimitive.Item
			className={cn(
				"relative flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-highlighted:bg-accent [&_svg:not([class*='size-'])]:size-4 [&_svg]:shrink-0 [&_svg]:text-muted-foreground",
				variant === 'destructive' &&
					'text-destructive data-highlighted:bg-destructive/8 [&_svg]:text-destructive',
				className
			)}
			data-slot="menu-item"
			{...props}
		/>
	);
}

export function MenuSeparator({
	className,
	...props
}: React.ComponentProps<typeof SeparatorPrimitive>): React.ReactElement {
	return (
		<SeparatorPrimitive
			className={cn('-mx-1 my-1 h-px bg-border', className)}
			data-slot="menu-separator"
			{...props}
		/>
	);
}

export { MenuPrimitive };
