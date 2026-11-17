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

package me.zodac.folding.client.java.request;

import static me.zodac.folding.api.util.EncodingUtils.encodeBasicAuthentication;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.Collection;
import lombok.AccessLevel;
import lombok.AllArgsConstructor;
import me.zodac.folding.api.tc.Hardware;
import me.zodac.folding.api.util.StringUtils;
import me.zodac.folding.rest.api.exception.FoldingRestException;
import me.zodac.folding.rest.api.header.ContentType;
import me.zodac.folding.rest.api.header.RestHeader;
import me.zodac.folding.rest.api.tc.request.HardwareRequest;
import me.zodac.folding.rest.util.RestUtilConstants;


@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class HardwareRequestSender {

    private final String hardwareUrl;

    
    public static HardwareRequestSender createWithUrl(final String foldingUrl) {
        final String hardwareUrl = foldingUrl + "/hardware";
        return new HardwareRequestSender(hardwareUrl);
    }

    
    public HttpResponse<String> getAll() throws FoldingRestException {
        return getAll(null);
    }

    
    public HttpResponse<String> getAll(final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(hardwareUrl))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get all hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get all hardware", e);
        }
    }

    
    public HttpResponse<String> get(final int hardwareId) throws FoldingRestException {
        return get(hardwareId, null);
    }

    
    public HttpResponse<String> get(final int hardwareId, final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(hardwareUrl + '/' + hardwareId))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get hardware", e);
        }
    }

    
    public HttpResponse<String> get(final String hardwareName) throws FoldingRestException {
        return get(hardwareName, null);
    }

    
    public HttpResponse<String> get(final String hardwareName, final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(hardwareUrl + "/fields?hardwareName=" + hardwareName))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get hardware", e);
        }
    }

    
    public HttpResponse<String> create(final HardwareRequest hardware) throws FoldingRestException {
        return create(hardware, null, null);
    }

    
    public HttpResponse<String> create(final HardwareRequest hardware, final String userName, final String password) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .POST(HttpRequest.BodyPublishers.ofString(RestUtilConstants.GSON.toJson(hardware)))
            .uri(URI.create(hardwareUrl))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to create hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to create hardware", e);
        }
    }

    
    public HttpResponse<String> createBatchOf(final Collection<HardwareRequest> batchOfHardware) throws FoldingRestException {
        return createBatchOf(batchOfHardware, null, null);
    }

    
    public HttpResponse<String> createBatchOf(final Collection<HardwareRequest> batchOfHardware, final String userName, final String password)
        throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .POST(HttpRequest.BodyPublishers.ofString(RestUtilConstants.GSON.toJson(batchOfHardware)))
            .uri(URI.create(hardwareUrl + "/batch"))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to create batch of hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to create batch of hardware", e);
        }
    }

    
    public HttpResponse<String> update(final int hardwareId, final HardwareRequest hardware) throws FoldingRestException {
        return update(hardwareId, hardware, null, null);
    }

    
    public HttpResponse<String> update(final int hardwareId, final HardwareRequest hardware, final String userName, final String password)
        throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .PUT(HttpRequest.BodyPublishers.ofString(RestUtilConstants.GSON.toJson(hardware)))
            .uri(URI.create(hardwareUrl + '/' + hardwareId))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to update hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to update hardware", e);
        }
    }

    
    public HttpResponse<Void> delete(final int hardwareId) throws FoldingRestException {
        return delete(hardwareId, null, null);
    }

    
    public HttpResponse<Void> delete(final int hardwareId, final String userName, final String password) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .DELETE()
            .uri(URI.create(hardwareUrl + '/' + hardwareId))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.discarding());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to delete hardware", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to delete hardware", e);
        }
    }
}
