import React from 'react';

export default class ErrorBoundary extends React.Component {
    constructor(props) {
      super(props);
      this.state = { hasError: false };
    }
  
    static getDerivedStateFromError(error) {
      return { hasError: true };
    }
  
    componentDidCatch(error, info) {
      console.error(error, info.componentStack);
    }
  
    render() {
      if (this.state.hasError) {
        // You can render any custom fallback UI
        return <p>This component has failed.</p>;
      }
  
      return this.props.children;
    }
  }