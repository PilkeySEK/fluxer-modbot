import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
	if (locals.user === null) {
		throw redirect(302, '/auth/initiate');
	}

	// NonNullable because we check for === null above
	return { user: locals.user as NonNullable<typeof locals.user> };
};
