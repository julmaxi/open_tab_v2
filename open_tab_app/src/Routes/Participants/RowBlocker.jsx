import { uniqueId } from "lodash";
import React from "react";

export class RowBlockManager {
    constructor() {
        this.blocked = new Set();

        this.block = this.blockInner.bind(this);
    }

    blockInner() {
        let lease = new BlockLease(this);
        this.blocked.add(lease.id);
        return lease;
    }

    unblock(leaseId) {
        this.blocked.delete(leaseId);
    }

    isBlocked() {
        return this.blocked.size > 0;
    }

    clear() {
        this.blocked.clear();
    }
}

export const RowBlockerContext = React.createContext(new RowBlockManager());

export class BlockLease {
    constructor(context) {
        this.unlockFn = context.unblock.bind(context);
        this.id = uniqueId();
    }

    unblock() {
        this.unlockFn(this.id)
    }
}
