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
import me.zodac.folding.api.util.StringUtils;
import me.zodac.folding.rest.api.exception.FoldingRestException;
import me.zodac.folding.rest.api.header.ContentType;
import me.zodac.folding.rest.api.header.RestHeader;
import me.zodac.folding.rest.api.tc.request.UserRequest;
import me.zodac.folding.rest.util.RestUtilConstants;


@AllArgsConstructor(access = AccessLevel.PRIVATE)
public final class UserRequestSender {

    private final String usersUrl;

    
    public static UserRequestSender createWithUrl(final String foldingUrl) {
        final String usersUrl = foldingUrl + "/users";
        return new UserRequestSender(usersUrl);
    }

    
    public HttpResponse<String> getAll() throws FoldingRestException {
        return getAll(null);
    }

    
    public HttpResponse<String> getAll(final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(usersUrl))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get all users", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get all users", e);
        }
    }

    
    public HttpResponse<String> get(final int userId) throws FoldingRestException {
        return get(userId, null);
    }

    
    public HttpResponse<String> get(final int userId, final String entityTag) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(usersUrl + '/' + userId))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get user with passkey", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get user with passkey", e);
        }
    }

    
    public HttpResponse<String> getWithPasskey(final int userId, final String userName, final String password) throws FoldingRestException {
        return getWithPasskey(userId, null, userName, password);
    }

    
    public HttpResponse<String> getWithPasskey(final int userId, final String entityTag, final String userName, final String password)
        throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .GET()
            .uri(URI.create(usersUrl + '/' + userId + "/passkey"))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNotBlank(entityTag)) {
            requestBuilder.header(RestHeader.IF_NONE_MATCH.headerName(), entityTag);
        }

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to get user with passkey", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to get user with passkey", e);
        }
    }

    
    public HttpResponse<String> create(final UserRequest user) throws FoldingRestException {
        return create(user, null, null);
    }

    
    public HttpResponse<String> create(final UserRequest user, final String userName, final String password) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .POST(HttpRequest.BodyPublishers.ofString(RestUtilConstants.GSON.toJson(user)))
            .uri(URI.create(usersUrl))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to create user", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to create user", e);
        }
    }

    
    public HttpResponse<String> createBatchOf(final Collection<UserRequest> batchOfUsers) throws FoldingRestException {
        return createBatchOf(batchOfUsers, null, null);
    }

    
    public HttpResponse<String> createBatchOf(final Collection<UserRequest> batchOfUsers, final String userName, final String password)
        throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .POST(HttpRequest.BodyPublishers.ofString(RestUtilConstants.GSON.toJson(batchOfUsers)))
            .uri(URI.create(usersUrl + "/batch"))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to create batch of users", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to create batch of users", e);
        }
    }

    
    public HttpResponse<String> update(final int userId, final UserRequest user) throws FoldingRestException {
        return update(userId, user, null, null);
    }

    
    public HttpResponse<String> update(final int userId, final UserRequest user, final String userName, final String password)
        throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .PUT(HttpRequest.BodyPublishers.ofString(RestUtilConstants.GSON.toJson(user)))
            .uri(URI.create(usersUrl + '/' + userId))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to update user", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to update user", e);
        }
    }

    
    public HttpResponse<Void> delete(final int userId) throws FoldingRestException {
        return delete(userId, null, null);
    }

    
    public HttpResponse<Void> delete(final int userId, final String userName, final String password) throws FoldingRestException {
        final HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()
            .DELETE()
            .uri(URI.create(usersUrl + '/' + userId))
            .header(RestHeader.CONTENT_TYPE.headerName(), ContentType.JSON.contentType());

        if (StringUtils.isNeitherBlank(userName, password)) {
            requestBuilder.header(RestHeader.AUTHORIZATION.headerName(), encodeBasicAuthentication(userName, password));
        }

        final HttpRequest request = requestBuilder.build();

        try {
            return RestUtilConstants.HTTP_CLIENT.send(request, HttpResponse.BodyHandlers.discarding());
        } catch (final InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new FoldingRestException("Error sending HTTP request to delete user", e);
        } catch (final IOException e) {
            throw new FoldingRestException("Error sending HTTP request to delete user", e);
        }
    }
}
