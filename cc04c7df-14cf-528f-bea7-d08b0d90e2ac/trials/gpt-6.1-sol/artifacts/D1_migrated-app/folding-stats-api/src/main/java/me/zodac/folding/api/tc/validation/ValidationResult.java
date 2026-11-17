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

package me.zodac.folding.api.tc.validation;

import java.util.Collection;
import java.util.Collections;
import java.util.List;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.NoArgsConstructor;
import lombok.Setter;
import lombok.ToString;
import me.zodac.folding.api.RequestPojo;
import me.zodac.folding.api.ResponsePojo;


@AllArgsConstructor(access = AccessLevel.PRIVATE)
@Getter
@EqualsAndHashCode
@ToString(doNotUseGetters = true)
public class ValidationResult<E extends ResponsePojo> {

    private final boolean success;
    private final E output;
    private final ValidationFailureType validationFailureType;
    private final ValidationFailure validationFailure;

    
    public boolean isFailure() {
        return !success;
    }

    
    public Collection<String> getErrors() {
        return validationFailure == null ? Collections.emptyList() : validationFailure.getErrors();
    }

    
    public static <E extends ResponsePojo> ValidationResult<E> successful(final E output) {
        return new ValidationResult<>(true, output, ValidationFailureType.NONE, null);
    }

    
    public static <E extends ResponsePojo> ValidationResult<E> failure(final Object invalidObject, final Collection<String> errors) {
        final ConstraintFailure response = new ConstraintFailure(invalidObject, errors);
        return new ValidationResult<>(false, null, ValidationFailureType.BAD_REQUEST, response);
    }

    
    public static <E extends ResponsePojo> ValidationResult<E> nullObject() {
        final ConstraintFailure response = new ConstraintFailure(null, List.of("Payload is null"));
        return new ValidationResult<>(false, null, ValidationFailureType.BAD_REQUEST, response);
    }

    
    public static <E extends ResponsePojo> ValidationResult<E> conflictingWith(final RequestPojo invalidObject,
                                                                               final ResponsePojo conflictingObject,
                                                                               final Collection<String> conflictingAttributes) {
        final ConflictFailure response = new ConflictFailure(
            invalidObject,
            List.of(conflictingObject),
            List.of(String.format("Payload conflicts with an existing object on: %s", conflictingAttributes))
        );
        return new ValidationResult<>(false, null, ValidationFailureType.CONFLICT, response);
    }

    
    public static <E extends ResponsePojo> ValidationResult<E> usedBy(final ResponsePojo invalidObject, final Collection<?> usedBy) {
        final ConflictFailure response = new ConflictFailure(invalidObject, usedBy, List.of("Payload is used by an existing object"));
        return new ValidationResult<>(false, null, ValidationFailureType.CONFLICT, response);
    }

    
    @NoArgsConstructor
    @AllArgsConstructor(access = AccessLevel.PRIVATE)
    @Getter
    @Setter
    public static class ConstraintFailure implements ValidationFailure {

        private Object invalidObject;
        private Collection<String> errors;
    }

    
    @NoArgsConstructor
    @AllArgsConstructor(access = AccessLevel.PRIVATE)
    @Getter
    @Setter
    public static class ConflictFailure implements ValidationFailure {

        private Object invalidObject;
        private Collection<?> conflictingObjects;
        private Collection<String> errors;
    }
}