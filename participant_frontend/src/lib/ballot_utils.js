
/**
 * @param {number[]} scores
 */
export function computeScoreTotal(scores) {
    let assignedScores = scores.filter(
        (score) => score !== null && score !== undefined
    );
    let sum = assignedScores.reduce((total, score) => {
        return total + score;
    }, 0);
    return assignedScores.length > 0 ? sum / assignedScores.length : null;
}

/**
 * @param {string} role
 */
export function roleToColor(role) {
    switch (role) {
        case "government":
            return "bg-green-200";
        case "opposition":
            return "bg-orange-200";
        case "non_aligned":
            return "bg-violet-200";
        default:
            return "bg-gray-500";
    }
}