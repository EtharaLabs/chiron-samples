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

package me.zodac.folding.client.java.response;

import com.google.gson.reflect.TypeToken;
import java.lang.reflect.Type;
import java.net.http.HttpResponse;
import java.util.Collection;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.rest.util.RestUtilConstants;


public final class HardwareResponseParser {

    private HardwareResponseParser() {

    }

    
    public static Collection<Hardware> getAll(final HttpResponse<String> response) {
        final Type collectionType = new TypeToken<Collection<Hardware>>() {
        }.getType();
        return RestUtilConstants.GSON.fromJson(response.body(), collectionType);
    }

    
    public static Hardware get(final HttpResponse<String> response) {
        return RestUtilConstants.GSON.fromJson(response.body(), Hardware.class);
    }

    
    public static Hardware create(final HttpResponse<String> response) {
        return RestUtilConstants.GSON.fromJson(response.body(), Hardware.class);
    }

    
    public static Hardware update(final HttpResponse<String> response) {
        return RestUtilConstants.GSON.fromJson(response.body(), Hardware.class);
    }
}