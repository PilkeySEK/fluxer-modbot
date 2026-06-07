import type { ApiRes } from '$lib';
import type { RouteParams } from '../../../routes/dashboard/manage/[guild_id]/$types';

export type DashboardPagesProps = {
	userData: ApiRes<'/users/@me'>;
	currentGuildData: ApiRes<'/guilds/{guild_id}'>;
	fetchedGuildData: ApiRes<'/guilds/{guild_id}'>;
	fluxerGuildData: ApiRes<'/users/@me/guilds'>[number];
	params: RouteParams;
};
