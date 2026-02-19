FROM nginx:alpine

# Install apache2-utils for htpasswd
RUN apk add --no-cache apache2-utils

# Build arguments for credentials
ARG ADMIN_USERNAME=admin
ARG ADMIN_PASSWORD=changeme

# Generate htpasswd file
RUN htpasswd -cb /etc/nginx/.htpasswd "${ADMIN_USERNAME}" "${ADMIN_PASSWORD}"

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy landing page
COPY landing /usr/share/nginx/html/landing

# Expose ports
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
