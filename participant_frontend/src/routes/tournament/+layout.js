/** @type {import('./$types').PageLoad} */
export function load({ params }) {
	return {
		tournamentId: params.tournament_id,
	};
}