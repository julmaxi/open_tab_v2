import { useEffect, useState, memo } from "react";
import { ISSUE_COLORS_BORDER, ISSUE_COLORS_BG, SWAP_ISSUE_GRADIENTS, severityToBucket } from "./Clashes";
import { useSpring, animated } from "react-spring";
import { ClashIndicator, IssueList } from "./ClashIndicator";


export function DragBox(props) {
  let highlightedIssues = props.highlightedIssues || [];
  let sortedIssues = highlightedIssues.sort((a, b) => b.severity - a.severity);
  let maxIssueSeverity = sortedIssues.length > 0 ? sortedIssues[0].severity : 0;
  let severityBucket = severityToBucket(maxIssueSeverity);
  let issueColor = ISSUE_COLORS_BORDER[severityBucket];
  let [isHovering, setIsHovering] = useState(false);

  useEffect(() => {
    if (isHovering) {
      let timer = setTimeout(() => {
        props.onHighlightIssues(isHovering, true);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [isHovering]);

  let expandIssues = props.expandIssues;

  let swapHighlightSeverity = props.swapHighlightSeverity;
  let swapIssueColor = null;
  if (swapHighlightSeverity !== null) {
    swapIssueColor = SWAP_ISSUE_GRADIENTS[swapHighlightSeverity]
  }

  const [animationProps, api] = useSpring(
    () => ({
      from: { opacity: 0 },
      to: { opacity: props.highlightedIssues?.length > 0 ? 1 : 0 },
    }),
    [props.highlightedIssues?.length]
  );

  return <div
    className={`relative flex bg-gray-100 min-w-[14rem] p-1 rounded overflow-clip`}
    style={{
      background: swapIssueColor
    }}
  >
    <div className="flex-1 min-w-0">
      {props.children}
    </div>
    <div className="flex items-center">
      <ClashIndicator issues={props.issues} onHover={(isHovering) => {
        props.onHighlightIssues(isHovering, false);
        setIsHovering(isHovering);
      }} />
    </div>
    {props.highlightedIssues?.length > 0 ? <animated.div style={animationProps} className={`absolute w-full h-full top-0 left-0 border-4 rounded text-white ${issueColor}`}>
      <div className={`absolute top-0 right-0 text-xs p-0.5 rounded-bl ${ISSUE_COLORS_BG[severityBucket]}`}>
        <IssueList highlightedIssues={props.highlightedIssues} expandIssues={expandIssues} />
      </div>
    </animated.div> : []}
  </div>
}