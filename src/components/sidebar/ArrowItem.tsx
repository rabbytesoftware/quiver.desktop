import type { ArrowEntry } from '@/domain/arrow'
import { useUIStore } from '@/store/ui'

interface Props {
  arrow: ArrowEntry
}

export function ArrowItem({ arrow }: Props) {
  const selectedNamespace = useUIStore((s) => s.selectedNamespace)
  const selectArrow = useUIStore((s) => s.selectArrow)
  const isSelected = selectedNamespace === arrow.namespace

  return (
    <div
      className={`group flex h-[34px] flex-shrink-0 cursor-pointer select-none items-center gap-2 px-2 ${
        isSelected ? 'bg-sidebar-foreground' : 'hover:bg-sidebar-accent'
      }`}
      onClick={() => selectArrow(arrow.namespace)}
    >
      {/* Icon placeholder */}
      <div className="h-4 w-4 flex-shrink-0 rounded-sm bg-[#F5A623]" />

      <div className="flex min-w-0 flex-col">
        <span
          className={`truncate text-[11px] font-medium leading-[1.4] ${
            isSelected ? 'text-sidebar' : 'text-sidebar-foreground/80'
          }`}
        >
          {arrow.name}
        </span>
        <span
          className={`truncate text-[9px] leading-[1.3] ${
            isSelected ? 'block text-sidebar/60' : 'hidden text-sidebar-foreground/40 group-hover:block'
          }`}
        >
          {arrow.namespace}
        </span>
      </div>
    </div>
  )
}
