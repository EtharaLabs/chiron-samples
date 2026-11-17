/*
 * MIT License
 *
 * Copyright (c) 2021-2022 zodac.me
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package me.zodac.folding.api.state;

import static me.zodac.folding.api.state.OperationType.READ;
import static me.zodac.folding.api.state.OperationType.WRITE;

import java.util.EnumSet;
import java.util.List;


public enum SystemState {

    
    AVAILABLE(READ, WRITE),

    
    RESETTING_STATS,

    
    STARTING,

    
    UPDATING_STATS(READ),

    
    WRITE_EXECUTED(READ, WRITE);

    private final EnumSet<OperationType> permittedOperationTypes;

    
    SystemState(final OperationType... operationTypes) {
        permittedOperationTypes = operationTypes.length == 0 ? EnumSet.noneOf(OperationType.class) : EnumSet.copyOf(List.of(operationTypes));
    }

    
    public boolean isReadBlocked() {
        return !permittedOperationTypes.contains(READ);
    }

    
    public boolean isWriteBlocked() {
        return !permittedOperationTypes.contains(WRITE);
    }
}
