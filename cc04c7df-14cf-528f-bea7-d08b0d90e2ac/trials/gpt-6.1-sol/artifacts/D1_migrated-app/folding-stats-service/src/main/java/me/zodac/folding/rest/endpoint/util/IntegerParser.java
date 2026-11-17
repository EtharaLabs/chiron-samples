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

package me.zodac.folding.rest.endpoint.util;

import org.springframework.web.bind.annotation.*;

import static me.zodac.folding.rest.response.Responses.badRequest;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

public final class IntegerParser {

    private static final Logger LOGGER = LogManager.getLogger();

    private IntegerParser() {

    }

    public static IdResult parsePositive(final String integer) {
        try {
            final int parsedId = Integer.parseInt(integer);
            if (parsedId < 0) {
                final String errorMessage = String.format("The ID '%s' is out of range", parsedId);
                LOGGER.error(errorMessage);
                return IdResult.failure(badRequest(errorMessage));
            }

            return IdResult.success(parsedId);
        } catch (final NumberFormatException e) {
            final String errorMessage = String.format("The ID '%s' is not a valid format", integer);
            LOGGER.error(errorMessage);
            return IdResult.failure(badRequest(errorMessage));
        }
    }
}
