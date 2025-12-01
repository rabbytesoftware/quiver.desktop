'use client';

import { cn } from '@/lib/utils';

import { Form as FormPrimitive } from '@base-ui-components/react/form';

function Form({ className, ...props }: FormPrimitive.Props) {
	return <FormPrimitive className={cn('flex w-full flex-col gap-4', className)} data-slot="form" {...props} />;
}

export { Form };
