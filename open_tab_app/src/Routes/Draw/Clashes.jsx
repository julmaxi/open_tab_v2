
export function severityToBucket(severity) {
    if (severity >= 75) {
        return "high";
    } else if (severity >= 50) {
        return "mid";
    } else if (severity >= 25) {
        return "low";
    } else {
        return "misc";
    }
}

export function bucketIssuesBySeverity(issues) {
    let issueBuckets = issues.reduce((acc, issue) => {
        let bucket = severityToBucket(issue.severity);
        acc[bucket].push(issue);
        return acc;
    }, { misc: [], low: [], mid: [], high: [] });
    return issueBuckets;
}


export const ISSUE_COLORS_BG = {
    neutral: "bg-gray-100",
    none: "bg-green-500",
    misc: "bg-gray-500",
    low: "bg-blue-500",
    mid: "bg-yellow-500",
    high: "bg-red-500"
}

export const ISSUE_COLORS_BORDER = {
    misc: "border-gray-500",
    low: "border-blue-500",
    mid: "border-yellow-500",
    high: "border-red-500"
}

export const SWAP_BASE_COLORS = {
    neutral: null,
    none: [34, 197, 94],
    misc: [107, 114, 128],
    low: [34, 197, 94],
    mid: [234, 179, 8],
    high: [239, 68, 68]
}


export const SWAP_ISSUE_GRADIENTS = Object.fromEntries(Object.entries(SWAP_BASE_COLORS).map(
    ([key, color]) => {
        if (color === null) {
            return [key, null];
        }
        let [r, g, b] = color;
        let r2 = Math.min(255, r + 20);
        let g2 = Math.min(255, g + 20);
        let b2 = Math.min(255, b + 20);

        return [key, `repeating-linear-gradient(45deg, rgb(${r} ${g} ${b}), rgb(${r} ${g} ${b}) 20px,rgb(${r2} ${g2} ${b2}) 20px, rgb(${r2} ${g2} ${b2}) 40px)`]
    }
));
export function find_issues_with_target(ballot, target_uuid) {
  return {
    "government": ballot.government !== null ? filter_issues_by_target(ballot.government.issues, target_uuid) : [],
    "opposition": ballot.opposition !== null ? filter_issues_by_target(ballot.opposition.issues, target_uuid) : [],
    "adjudicators": ballot.adjudicators !== null ? ballot.adjudicators.map(adj => filter_issues_by_target(adj.issues, target_uuid)) : [],
    "non_aligned_speakers": ballot.non_aligned_speakers !== null ? ballot.non_aligned_speakers.map(speaker => filter_issues_by_target(speaker.issues, target_uuid)) : []
  };
}
function filter_issues_by_target(issues, target_uuid) {
  return issues.filter((i) => i.target.uuid === target_uuid);
}export function getMaxSeverityFromEvaluationResult(result) {
  let allIssues = [];
  for (let issues of Object.values(result)) {
    for (let elem of issues) {
      if (Array.isArray(elem)) {
        allIssues.push(...elem);
      }
      else {
        allIssues.push(elem);
      }
    }
  }
  let maxSeverity = Math.max(0, ...allIssues.map((issue) => issue.severity));
  return maxSeverity;
}

