import type { CSSProperties, JSX } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

import type { CollectionDetail } from '@/domain/collection';
import { ArrowIcon } from '@/features/sidebar/components/arrows/arrow-icon';
import { useFollowCollection, useUnfollowCollection } from '@/lib/core-store/mutations/collection';
import { cssUrl } from '@/lib/css';

import '../styles/collection.css';

interface CollectionHeroProps {
	collection: CollectionDetail;
	unresolvedCount?: number;
	onUnresolvedClick?: () => void;
}

export function CollectionHero({
	collection,
	unresolvedCount = 0,
	onUnresolvedClick,
}: CollectionHeroProps): JSX.Element {
	const follow = useFollowCollection();
	const unfollow = useUnfollowCollection();

	const toggleFollow = () => {
		if (collection.followed) unfollow.mutate({ namespace: collection.namespace });
		else follow.mutate({ namespace: collection.namespace });
	};

	const banner = collection.media.banner;
	const bannerStyle: CSSProperties | undefined = banner ? { backgroundImage: cssUrl(banner) } : undefined;

	return (
		<div className="collection-hero">
			<div className={`collection-hero-row${banner ? '' : ' no-banner'}`}>
				{banner && <div className="collection-banner" style={bannerStyle} aria-hidden="true" />}
				<div className="collection-hero-left">
					<div className="collection-hero-identity">
						<span className="collection-badge-wrap" style={{ '--icon': '44px' } as CSSProperties}>
							<ArrowIcon
								namespace={collection.namespace}
								name={collection.name}
								icon={collection.media.icon ?? null}
							/>
						</span>
						<div className="collection-title-block">
							<div className="collection-title">{collection.name}</div>
							<div className="collection-namespace">{collection.namespace}</div>
						</div>
					</div>

					<p className="collection-description">{collection.description}</p>

					<div className="collection-meta">
						<span className="count">{collection.arrowCount} arrows</span>
						{collection.maintainers.length > 0 && (
							<>
								<span className="dot">·</span>
								<span>maintained by {collection.maintainers.join(', ')}</span>
							</>
						)}
						{unresolvedCount > 0 && (
							<>
								<span className="dot">·</span>
								<Badge
									className="collection-unresolved-flag"
									render={<button onClick={onUnresolvedClick} type="button" />}
									variant="error"
								>
									{unresolvedCount} unresolved
								</Badge>
							</>
						)}
					</div>

					<div className="collection-actions">
						<Button
							variant={collection.followed ? 'outline' : 'default'}
							onClick={toggleFollow}
							disabled={follow.isPending || unfollow.isPending}
						>
							{collection.followed ? 'Following' : 'Follow'}
						</Button>
					</div>
				</div>
			</div>
		</div>
	);
}
