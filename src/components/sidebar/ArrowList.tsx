import { useMemo } from 'react'
import { useArrowStore } from '@/lib/core-store'
import { ArrowItem } from './ArrowItem'

export function ArrowList() {
  const arrowsMap = useArrowStore((s) => s.arrows)
  const arrows = useMemo(() => Array.from(arrowsMap.values()), [arrowsMap])

  return (
    <div className="flex flex-1 flex-col overflow-y-auto">
      {arrows.map((arrow) => (
        <ArrowItem key={arrow.namespace} arrow={arrow} />
      ))}
    </div>
  )
}
